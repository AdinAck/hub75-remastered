#!/bin/bash

set -euxo pipefail

cargo build --features hal-02
cargo build --features hal-1
