ifeq ($(version),)
version=$(shell cat VERSION)
ifeq ($(version),)
	$(error version is not set)
endif
endif

version:=$(version)-$(shell git rev-parse --short HEAD)

project=$(shell ./project-name.sh)
docker_image=$(shell ./docker-name.sh)

AARCH64=aarch64-unknown-linux-musl
AMD64=x86_64-unknown-linux-musl

.PHONY: all all-docker manifest

all: all-docker

# docker.target
docker.%:
	cross build --release --target $*
	# hack to replace entrypoint	
	mkdir -p buildtmp
	sed -e 's,ENTRYPOINT.*,ENTRYPOINT ["/$(project)"],g' Dockerfile >buildtmp/Dockerfile.$(project)
	cp target/$*/release/$(project) buildtmp
	docker build -t $(docker_image):$(version)-$* --build-arg BIN=$(project) -f buildtmp/Dockerfile.$(project) buildtmp
	rm -rf buildtmp

all-docker: docker.$(AARCH64) docker.$(AMD64)

manifest: all-docker
	docker push $(docker_image):$(version)-$(AARCH64)
	docker push $(docker_image):$(version)-$(AMD64)
	docker manifest create --amend $(docker_image):$(version)\
		$(docker_image):$(version)-$(AARCH64)\
		$(docker_image):$(version)-$(AMD64)
	docker manifest annotate --arch arm64 $(docker_image):$(version)\
		$(docker_image):$(version)-$(AARCH64)
	docker manifest annotate --arch amd64 $(docker_image):$(version)\
		$(docker_image):$(version)-$(AMD64)
	docker manifest push --purge $(docker_image):$(version)
