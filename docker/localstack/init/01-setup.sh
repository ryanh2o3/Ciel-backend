#!/bin/bash
set -euo pipefail

awslocal s3 mb s3://ciel-media || true
awslocal sqs create-queue --queue-name ciel-media-jobs || true
