#!/bin/python3

import os.path
import argparse
import multiprocessing

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
parser.add_argument("output_stem", help="where to place the output files")

args = parser.parse_args()

assert os.path.exists(args.input), "input file does not exist"

with open(args.input) as fin:
    content = fin.read()
recent = content.split("\n\n")[-3]
assert recent is not None, "failed to split content"
summary = content.split("\n\n")[-2]
assert summary is not None, "failed to split content"

def maxLen(x: str) -> int:
    return max(map(lambda y: len(ansi_escape.sub("", y)), x.splitlines()))

ansiToRender(content, f"{args.output_stem}.col.png", title=args.input, width=maxLen(content), theme="./theme.yml")
ansiToRender(recent, f"{args.output_stem}_tab.png", title=f"most recent table of {args.input}", width=maxLen(recent), theme="./theme.yml")
ansiToRender(summary, f"{args.output_stem}_sum.png", title=f"summary table of {args.input}", width=maxLen(summary), theme="./theme.yml")

multiprocessing.Pool().imap(
    lambda x: ansiToRender(x[1], f"{args.output_stem}_{x[0]}.png", title=f"{x[0]}th table of {args.input}", width=maxLen(x[1]), theme="./theme.yml"),
    enumerate(content.split("\n\n")[:-2])
)
