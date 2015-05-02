#!/bin/bash

if [ -z "$USE_GLOBAL_INSTALL" ]; then
	CLANG_OPTS="-L target/debug -I src/"
	export LD_LIBRARY_PATH="target/debug"
fi

function run_test {
    local file=$1;
    clang -g -O0 "tests/$file.c" ${CLANG_OPTS} -l rcimmixcons -o "target/$file" || return 1;
    "./target/$file" || return 2;
    valgrind "./target/$file" || return 3;
    return 0;
}

function main {
	local code=0;
	for path in tests/$1*.c; do
		local file=`basename "$path" | sed 's/\.c//'`;
		echo -n "Running test $file.."
		output=$(run_test $file 2>&1);
		if [ $? -ne 0 ]; then
			echo "fail";
			echo -e "\n$output\n" | tee "target/log_$file";
			code=1;
		else
			echo "ok";
		fi;
	done;
	exit $code;
}

main $@;
