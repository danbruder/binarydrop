#! /bin/bash

set -e
IP=167.172.25.50

# echo "Building the frontend..."
# cd assets && npm run build && cd ..

echo "Building the backend..."
TARGET_CC=x86_64-linux-musl-gcc cargo build --release --target x86_64-unknown-linux-musl

echo "Pushing..."
scp ./target/x86_64-unknown-linux-musl/release/bindrop bindrop@$IP:/opt/bindrop/bindrop.new

echo "Deploying..."
ssh root@$IP -t 'systemctl stop bindrop'
ssh bindrop@$IP -t 'mv /opt/bindrop/bindrop.new /opt/bindrop/bindrop'
ssh root@$IP -t 'systemctl restart bindrop'

echo "Done!"
