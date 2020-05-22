#!/usr/bin/env bash

# Usage:
# PWD should be root of workspace (project)
# run: ./test-shutdown.sh <executable name>
# There `executable name` can be "dvm" or "compiler"

echo "Testing SIGTERM catching for $1"

# preparing tmp dir
# should fail if wrong environment (such as PWD)
CUR_TMP_DIR=./tests/tmp
mkdir $CUR_TMP_DIR

STDOUT_LOG_PATH=$CUR_TMP_DIR/$1.log
STDERR_LOG_PATH=$CUR_TMP_DIR/$1.error.log
KILL_EXIT_CODE_PATH=$CUR_TMP_DIR/$1.kill.exit-code


EXPECTED_EXIT_CODE=130
EXPECTED_KILL_EXIT_CODE=0
EXPECTED_STDERR_LOG_SIZE=0

# build
# cargo build --bin $1

# run with verbosity and log redirection
./target/debug/$1 -v > $STDOUT_LOG_PATH 2>$STDERR_LOG_PATH &
EXECUTABLE_PID=$!
echo "run $1, PID: $EXECUTABLE_PID"


echo "killing-timer setted up for $EXECUTABLE_PID"
(sleep 3 && kill -TERM $EXECUTABLE_PID && echo "$?">$KILL_EXIT_CODE_PATH && echo "killed $EXECUTABLE_PID") &

wait "$EXECUTABLE_PID"
EXECUTABLE_EXIT_CODE=$?
EXECUTABLE_KILL_EXIT_CODE=`cat $KILL_EXIT_CODE_PATH`

echo "$1 exit-code: $EXECUTABLE_EXIT_CODE"
echo "kill exit-code: $EXECUTABLE_KILL_EXIT_CODE"

# check kill exit code:
if [ "$EXECUTABLE_KILL_EXIT_CODE" == "$EXPECTED_KILL_EXIT_CODE" ]; then
  echo "SIGTERM catched: OK"
else
  echo "SIGTERM not catched: ERR: $EXECUTABLE_KILL_EXIT_CODE != $EXPECTED_KILL_EXIT_CODE"
  exit 1
fi

# check the exit code:
if [ $EXECUTABLE_EXIT_CODE == $EXPECTED_EXIT_CODE ]; then
  echo "exit-code: OK"
else
  echo "exit-code: ERR: $EXECUTABLE_EXIT_CODE != $EXPECTED_EXIT_CODE"
  exit 1
fi

# check size of stderr:
STDERR_LOG=`cat $STDERR_LOG_PATH`
STDERR_LOG_SIZE=${#STDERR_LOG}
if [ $STDERR_LOG_SIZE == $EXPECTED_STDERR_LOG_SIZE ]; then
  echo "stderr size: OK"
else
  echo "stderr size: ERR: $STDERR_LOG_SIZE != $EXPECTED_STDERR_LOG_SIZE"
  exit 1
fi

# check size of stdout:
STDOUT_LOG=`cat $STDOUT_LOG_PATH`
STDOUT_LOG_SIZE=${#STDOUT_LOG}
if [ $STDOUT_LOG_SIZE -gt 0 ]; then
  echo "stdout size: OK"
else
  echo "stdout size: ERR: $STDOUT_LOG_SIZE but should be more than zero"
  exit 1
fi

# TODO: mb. find something like "Received signal TERM" in the stdout-log file?
