#!/bin/bash

cat Cargo.toml | grep 'name *=' | sed -e 's/^name *= *"\(.*\)" *$/\1/g'
