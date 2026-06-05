use anyhow::bail;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::{
    env, mem, process,
    sync::Arc,
    time::{Duration, Instant},
};
use structopt::StructOpt;
use tokio::{
    sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError},
    task::JoinHandle,
    time,
};
use url::Url;
use xedge::{SIGINT, SIGTERM};
use xompass_object_detection::{
    AsyncDetectorMut, Detection,
    detectors::{
        Grpc,
        grpc::{GrpcConfig, JpgBytes},
    },
};

#[derive(StructOpt)]
struct Opt {
    #[structopt(short = "t", long, help = "timeout for grpc in secs")]
    grpc_timeout_s: u64,
    #[structopt(
        long,
        help = "timeout for retrieving token in secs",
        default_value = "10"
    )]
    token_timeout_s: u64,
    #[structopt(
        long,
        help = "vsaas bridge api base url",
        default_value = "https://bridge.xompass.com"
    )]
    bridge_base: String,
    #[structopt(long, help = "credential id @ vsaas.ai")]
    token_from_credential: Option<String>,
    #[structopt(long, help = "one or more grpc urls")]
    grpc_url: Vec<Url>,
    #[structopt(long, help = "how many inputs can be processed concurrently")]
    backpressure: usize,
    #[structopt(
        long,
        help = "move license plate detections to dedicated sink 'isolated-license-plates'"
    )]
    isolate_license_plates: bool,
}

impl Opt {
    fn is_valid(&self) -> bool {
        if self.backpressure == 0 {
            eprintln!("backpressure: must be > 0");
            return false;
        }
        if self.grpc_url.is_empty() {
            eprintln!("grpc_url: must define at least 1");
            return false;
        }
        true
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum InputTimestamp {
    Epoch(i64),
    Chrono(DateTime<Utc>),
}

impl InputTimestamp {
    fn to_epoch(&self) -> i64 {
        match self {
            InputTimestamp::Epoch(epoch) => *epoch,
            InputTimestamp::Chrono(dt) => dt.timestamp_millis(),
        }
    }
}

#[derive(Deserialize)]
struct InputPayload {
    ts: InputTimestamp,
    asset_id: String,
}

#[derive(Serialize)]
struct Frame {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Serialize)]
struct Datum {
    class: String,
    probability: f64,
    frame: Frame,
}

#[derive(Debug)]
struct RelativeBbox;

impl Datum {
    fn try_from_detection(detection: Detection) -> Result<Datum, RelativeBbox> {
        let bbox = detection.bbox;
        if bbox.relative {
            return Err(RelativeBbox);
        }
        let frame = Frame {
            x: (bbox.left + bbox.width / 2.) as u32,
            y: (bbox.top + bbox.height / 2.) as u32,
            w: bbox.width as u32,
            h: bbox.height as u32,
        };
        Ok(Datum {
            class: detection.class.into_owned(),
            frame,
            probability: detection.prob,
        })
    }
}

#[derive(Serialize)]
struct OutputPayload<'a> {
    ts: i64,
    asset_id: &'a str,
    path: &'a str,
    data: Vec<Datum>,
}

struct ValidatedInput {
    data: Bytes,
    imfrom: usize,
    json: InputPayload,
}

impl ValidatedInput {
    fn image(&self) -> &[u8] {
        &self.data[self.imfrom..]
    }
}

fn validated_input(data: Bytes) -> Option<ValidatedInput> {
    let (imfrom, json) = {
        let (jsonb, imageb) = xompassutil::json_meta(&data)?;
        if imageb.is_empty() {
            error!("dropping input: empty image");
            return None;
        }
        let json: InputPayload = serde_json::from_slice(jsonb)
            .map_err(|e| error!("Invalid json metadata: {e:?}"))
            .ok()?;
        (jsonb.len(), json)
    };
    Some(ValidatedInput { data, imfrom, json })
}

async fn process_input(
    input: ValidatedInput,
    _permit: OwnedSemaphorePermit,
    timeout: Duration,
    mut grpc: Grpc,
    writer: xedge::Writer,
    isolate_license_plates: bool,
) {
    let jpg_bytes = JpgBytes(input.image().to_vec());
    let start = Instant::now();
    match time::timeout(timeout, grpc.detect(jpg_bytes)).await {
        Ok(Ok(detections)) => {
            let request_t = start.elapsed();
            let post = Instant::now();
            let mut transformed: Vec<_> = match detections
                .into_iter()
                .map(Datum::try_from_detection)
                .collect::<Result<_, _>>()
            {
                Ok(t) => t,
                Err(e) => {
                    error!("transform detections: {e:#?}");
                    return;
                }
            };
            let mut plate_detections = Vec::new();
            if isolate_license_plates {
                plate_detections = transformed
                    .extract_if(.., |datum| &datum.class == "license_plate")
                    .collect();
            }
            let output = OutputPayload {
                ts: input.json.ts.to_epoch(),
                asset_id: &input.json.asset_id,
                // NOTE: backwards-compatible
                path: "",
                data: transformed,
            };
            match serde_json::to_vec(&output) {
                Ok(mut payload) => {
                    payload.extend(input.image());
                    if let Err(e) = writer.write("detections", &payload).await {
                        error!("mqtt: {e:?}");
                    }
                    info!(
                        "grpc request={request_t:?} postprocess={:?}",
                        post.elapsed()
                    );
                }
                Err(e) => error!("output serialize: {e:?}"),
            }
            if !plate_detections.is_empty() {
                let output = OutputPayload {
                    ts: input.json.ts.to_epoch(),
                    asset_id: &input.json.asset_id,
                    // NOTE: backwards-compatible
                    path: "",
                    data: plate_detections,
                };

                match serde_json::to_vec(&output) {
                    Ok(mut payload) => {
                        payload.extend(input.image());
                        if let Err(e) = writer.write("isolated-license-plates", &payload).await {
                            error!("mqtt: {e:?}");
                        }
                    }
                    Err(e) => error!("output serialize: {e:?}"),
                }
            }
        }
        Ok(Err(grpc_error)) => error!("grpc: {grpc_error:?}"),
        Err(_timeout) => error!("grpc: timeout"),
    }
}

fn retrieve_token(
    credential_id: &str,
    device_token: &str,
    timeout: Duration,
    bridge_base: &str,
) -> anyhow::Result<String> {
    let sep = if bridge_base.ends_with("/") { "" } else { "/" };
    let url = format!("{bridge_base}{sep}api/Credentials");
    let filter = format!(r#"{{ "where": {{ "identifier": "{credential_id}" }} }}"#);
    info!("GET {url}");
    let mut req = ureq::get(url)
        .query("filter", filter)
        .header("device_token", device_token)
        .config()
        .timeout_global(Some(timeout))
        .http_status_as_error(true)
        .build()
        .call()?;
    #[derive(Deserialize)]
    struct Content {
        token: String,
    }
    #[derive(Deserialize)]
    struct Credential {
        content: Content,
    }
    let mut credentials: Vec<Credential> = req.body_mut().read_json()?;
    if credentials.is_empty() {
        bail!("credentials: no credential found with id {credential_id}");
    } else if credentials.len() > 1 {
        bail!("credentials: too many credentials found with id {credential_id}");
    } else {
        Ok(mem::take(&mut credentials[0].content.token))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let opt = Opt::from_args();
    if !opt.is_valid() {
        process::exit(1)
    }
    let mut token = None;
    if let Some(credential_id) = opt.token_from_credential {
        let Ok(device_token) = env::var("XEDGE_DEVICE_TOKEN") else {
            eprintln!("missing XEDGE_DEVICE_TOKEN");
            process::exit(1);
        };
        token = match retrieve_token(
            &credential_id,
            &device_token,
            Duration::from_secs(opt.token_timeout_s),
            &opt.bridge_base,
        ) {
            Ok(token) => Some(token),
            Err(e) => {
                error!("retrieve_token: {e:?}");
                process::exit(1)
            }
        };
    }
    log::info!("xedge v{}", xedge::version());
    let mut module = xedge::Module::from_env_with_signals([SIGINT, SIGTERM])
        .expect("xedge env vars should be set");
    let mut running_tasks: Vec<JoinHandle<()>> = vec![];
    let backpressure = Arc::new(Semaphore::new(opt.backpressure));
    let config = GrpcConfig {
        url: opt.grpc_url,
        // NOTE: netsize is used only for method netsize, which is not used by Grpc.
        // If resize feature is to be added, this should be changed.
        netsize: (0, 0),
        token,
    };
    let grpc = match Grpc::from_config(&config) {
        Ok(grpc) => grpc,
        Err(e) => {
            error!("grpc create: {e:?}");
            process::exit(1);
        }
    };
    loop {
        // clean running task registry of all finished ones
        running_tasks.retain(|task| !task.is_finished());
        match module.next_event().await {
            xedge::Event::IncomingParameters(_) => {}
            xedge::Event::IncomingGate(gate, data) => {
                if gate != "image" {
                    warn!("Expecting gate 'image'. Got '{gate}'.");
                    continue;
                }
                let permit = match backpressure.clone().try_acquire_owned() {
                    Ok(permit) => permit,
                    Err(TryAcquireError::NoPermits) => {
                        warn!("droping input: backpressure");
                        continue;
                    }
                    Err(TryAcquireError::Closed) => unreachable!(),
                };
                let Some(input) = validated_input(data) else {
                    error!("dropping invalid input");
                    continue;
                };
                let task = tokio::spawn(process_input(
                    input,
                    permit,
                    Duration::from_secs(opt.grpc_timeout_s),
                    grpc.clone(),
                    module.writer(),
                    opt.isolate_license_plates,
                ));
                running_tasks.push(task);
            }
            xedge::Event::IncomingTypesink(_typesink, _payload) => {}
            xedge::Event::Signal(sig) => {
                if sig == SIGINT || sig == SIGTERM {
                    info!("Waiting on all running tasks");
                    for task in running_tasks {
                        // NOTE: ignore cancelled join error
                        if let Err(join_err) = task.await
                            && let Ok(reason) = join_err.try_into_panic()
                        {
                            std::panic::resume_unwind(reason);
                        }
                    }
                    break;
                }
            }
        }
    }
}
