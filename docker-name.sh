#!/bin/bash

suffix=$(./project-name.sh | sed -e 's/xompass-//g')

echo xompassdevregistry.azurecr.io/$suffix
