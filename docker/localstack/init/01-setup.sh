#!/bin/bash
set -euo pipefail

awslocal s3 mb s3://lumine-media || true
awslocal sqs create-queue --queue-name lumine-media-jobs || true
