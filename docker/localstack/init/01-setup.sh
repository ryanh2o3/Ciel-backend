#!/bin/bash
set -euo pipefail

awslocal s3 mb s3://picshare-media || true
awslocal sqs create-queue --queue-name picshare-media-jobs || true
