#!/bin/bash

readarray -t arr < ts.dat
for line in "${arr[@]}" ; do
	touch -m -t "${line%% *}" "${line#* }"
done
