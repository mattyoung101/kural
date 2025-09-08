#!/bin/bash
set -eoux pipefail

cargo sqlx prepare
git add .sqlx
