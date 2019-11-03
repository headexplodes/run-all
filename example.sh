#!/bin/bash

# Simple testing script, example:
# cargo run -- -a alpha "./example.sh 3" -a beta "./example.sh 5" -a gamma "./example.sh 7"

# testnig signal handling on children
trap "echo \"Trapped SIGINT, exiting...\"" EXIT

COUNT="$1"

# testing stderr
echo "Will loop ${COUNT} time(s)" > /dev/stderr

for (( i = 1; i <= COUNT; i++ ))
do
   echo "Testing #$i..."
   sleep 1
done

# testing exit status display
exit $COUNT

