#!/bin/python3

import os.path
import argparse

from ansitoimg.render import ansiToRender

import re

# 7-bit C1 ANSI sequences
ansi_escape = re.compile(r'''
    \x1B  # ESC
    (?:   # 7-bit C1 Fe (except CSI)
        [@-Z\\-_]
    |     # or [ for CSI, followed by a control sequence
        \[
        [0-?]*  # Parameter bytes
        [ -/]*  # Intermediate bytes
        [@-~]   # Final byte
    )
''', re.VERBOSE)

parser = argparse.ArgumentParser()
parser.add_argument("input", help="the .col.out (text) file which shall be processed")
parser.add_argument("output_all", help="png file to write the complete log to")
parser.add_argument("output_recent", help="png file to write only the most recent table to")

args = parser.parse_args()

assert os.path.exists(args.input), "input file does not exist"

with open(args.input) as fin:
    content = fin.read()
recent = content.split("\n\n")[-3]
assert recent is not None, "failed to split content"

def maxLen(x: str) -> int:
    return max(map(lambda y: len(ansi_escape.sub("", y)), x.splitlines()))

ansiToRender(content, args.output_all, title=args.input, width=maxLen(content))
ansiToRender(recent, args.output_recent, title=f"most recent table of {args.input}", width=maxLen(recent))
