#!/bin/bash

echoerr() {
    >&2 echo "$@"
}

if [ -z "$TOKEN" ]; then
    echoerr "Missing TOKEN."
    exit 1
fi

echoerr "POST https://api.xompass.com/api/Credentials"
read -p "identifier: " identifier
read -sp "token: " token

curl -vXPOST "https://api.xompass.com/api/Credentials" -H "Content-Type: application/json" -H "Authorization: $TOKEN" -d '{
    "identifier": "'$identifier'",
    "name": "'$identifier'",
    "type": "AccessToken",
    "content": {
        "request": "headers",
        "url": "none",
        "base": "none",
        "token": "'$token'"
    }
}'
