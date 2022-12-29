from prettytable import PrettyTable
import itertools
import math
import yaml
import time
import argparse
from subprocess import Popen
from colorama import Fore, Style
from contextlib import ExitStack
# import sys

def binomial(a,b):
    return math.factorial(a) // math.factorial(b) // math.factorial(a-b)

def gen_poss(num):
    for ele in itertools.permutations(range(num)):
        yield tuple([e] for e in ele)

# TODO not implemented
def add_dup(g, d):
    # somewhat similar like someone_is_dup but the last element is always the same
    for ele in g:
        for p in ele:
            p.append(d)
            yield ele
            p.pop()

def someone_is_dup(g):
    for ele in g:
        # iterate over the positions for the duplication
        for p in ele[:-1]:
            # enforce ordering regarding the duplicats
            if p[0] < ele[-1][0]: continue
            # add possible duplicate
            p.append(ele[-1][0])
            # don't yield the last element which belongs to the duplication
            yield ele[:-1]
            # remove the duplcation to restore the same state as before
            p.pop()

def print_matching_to_file(matching, lutA, lutB, file):
    d = {lutA[i1] : sorted([lutB[x] for x in v]) for i1,v in enumerate(matching)}
    for k,v in sorted(d.items(), key=lambda x: x[0]):
        print(k, "->", v, end=" | ", file=file)
    print(file=file)

def print_map(x:tuple[tuple[float,...],int], stride:int, lutA:list, lutB:list, color:bool):
    m,total_left = x
    t = PrettyTable()
    t.field_names = [""] + lutB
    for r in range(len(m)//stride):
        if color:
            t.add_row([lutA[r], *(f"{Fore.GREEN}{x:03.4}{Style.RESET_ALL}" if 79 < x < 101 else f"{Fore.RED}{x:03.4}{Style.RESET_ALL}" if -1 < x < 1 else f"{x:03.4}"  for x in m[r*stride:r*stride+stride])])
        else:
            t.add_row([lutA[r], *(f"{x:03.4}" for x in m[r*stride:r*stride+stride])])
    print(t.get_string())
    print(total_left, "left ->", math.log2(total_left), "bits left")

def print_map_dot(x:tuple[tuple[float,...],int], stride:int, lutA:list[str], lutB:list[str], color:bool, dotTab):
    m,_ = x
    print("digraph structs { node[shape=plaintext] struct[label=<", file=dotTab)
    print('<table cellspacing="2" border="0" rows="*" columns="*">', file=dotTab)

    print("<tr>", file=dotTab)
    print("<td></td>", file=dotTab) # empty cell
    for y in lutB:
        print(f"<td><B>{y}</B></td>", file=dotTab)
    print("</tr>", file=dotTab)
    for r in range(len(m)//stride):
        print(f"<tr><td><B>{lutA[r]}</B></td>", file=dotTab)
        for y in m[r*stride:r*stride+stride]:
            print("<td>", file=dotTab, end="")
            if color and 79 < y < 101:
                print('<font color="darkgreen">', file=dotTab, end="")
            elif color and -1 < y < 1:
                print('<font color="red">', file=dotTab, end="")
            else:
                print('<font color="black">', file=dotTab, end="")
            print(f"{y:03.4}</font></td>", file=dotTab)
        print("</tr>", file=dotTab)
    print("</table>", file=dotTab)
    print(">];}", file=dotTab)

class Constraint:
    def __init__(self, type:str, num:int|float, comment:str, lights:int, map:dict[int,int], sizeA:int, sizeB:int, init:int=0):
        self.type           = type
        self.num            = num
        self.comment        = comment
        self.lights         = lights
        self.map            = map # TODO better type for this than a map? (probably a list)
        self.eliminated     = 0
        self.eliminated_tab = [init] * sizeB*sizeA
        self.stride = sizeB

    def fits(self, matching:tuple):
        l = 0
        for i1,i2 in self.map.items():
            if i2 in matching[i1]:
                l += 1
        return l == self.lights

    def eliminate(self, matching:tuple):
            # print(matching)
            for i1,v in enumerate(matching):
                for i2 in v:
                    # print(i1,i2)
                    self.eliminated_tab[i1*self.stride+i2] += 1
            self.eliminated += 1

    def apply_to_rem(self, rem:tuple[tuple,int]):
            tab,total = rem
            total -= self.eliminated
            # tab = list(map(lambda x: list(map(lambda y: (y[0] - y[1]), zip(*x))), zip(tab, self.eliminated_tab)))
            tab = tuple(map(lambda y: y[0]-y[1], zip(tab, self.eliminated_tab)))

            # self.tab_left   = list(map(lambda r: list(map(lambda x: x/total*100, r)), tab))
            self.tab_left = tuple(map(lambda r: r/total*100, tab))
            self.total_left = total

            tmp = 1-self.eliminated/(self.total_left+self.eliminated)
            # print(self.eliminated, self.total_left, tmp)
            self.entro = -math.log2(tmp) if tmp > 0 else None

            return tab,total

    def print_left(self, lutA, lutB, color):
        print(f"{self.lights} {self.type}#{self.num:02.1f} {self.comment}")
        d = {lutA[i1] : lutB[i2] for i1,i2 in self.map.items()}
        for k,v in sorted(d.items(), key=lambda x: x[0]):
            print(k, "->", v)
        print(f"-> I = {self.entro if self.entro else 'inf'}")
        print_map((self.tab_left,self.total_left), self.stride, lutA, lutB, color)

    def row(self, lutA, lutB):
        return [
                f"{self.type}#{self.num:02.1f}",
                self.lights,
                *(lutB[self.map[x]] if x in self.map else "" for x in range(len(lutA))),
                "",
                self.entro if self.entro else 'inf'
                ]

    def write_stats(self, mno, mbo, info):
        if self.type == "MB":
            print(f"{self.num*2-1} {math.log2(self.total_left)}", file=info)
            print(f"{self.num} {self.entro if self.entro else 'inf'}", file=mbo)
        elif self.type == "MN":
            print(f"{self.num*2} {math.log2(self.total_left)}", file=info)
            print(f"{self.num} {self.entro if self.entro else 'inf'}", file=mno)
        else:
            raise Exception("invalid type")

    @classmethod
    def parse(cls, data, lutAr, lutBr):
        # TODO write error messages
        assert isinstance(data, dict), ""
        assert all([x in data for x in ["type", "lights", "num", "comment", "map"]]), ""

        assert data["type"] in ["MB", "MN"], ""
        assert isinstance(data["num"], float) or isinstance(data["num"], int)
        assert isinstance(data["lights"], int)
        assert isinstance(data["comment"], str)
        assert isinstance(data["map"], dict)

        if data["type"] == "MB":
            assert data["lights"] == 0 or data["lights"] == 1, ""
        # TODO more assertions

        m = {lutAr[v1] : lutBr[v2] for v1,v2 in data["map"].items()}

        return cls(type=data["type"], num=data["num"], comment=data["comment"], lights=data["lights"], map=m, sizeA=len(lutAr), sizeB=len(lutBr))

class Game:
    def __init__(self, constraints:list[Constraint], lutA, lutAr, lutB, lutBr):
        self.constraints = constraints
        self.lutA        = lutA
        self.lutAr       = lutAr
        self.lutB        = lutB
        self.lutBr       = lutBr

    def filter_pred(self, matching:tuple):
        for c in self.constraints:
            if not c.fits(matching):
                c.eliminate(matching)
                return False
        return True

    def sim(self, color:bool, dot_bound:int, output_stem:str, print_matchings:str|None):
        assert len(self.lutA) == len(self.lutB)-1

        remaining,total,each = 0,0,0
        g = gen_poss(len(self.lutB))
        g = someone_is_dup(g)

        left_poss = []
        with ExitStack() as stack:
            if print_matchings:
                r_out = stack.enter_context(open(print_matchings, "w"))
            else:
                r_out = None
            for matching in g:
                if 0 in matching[0]: each += 1
                total += 1
                if self.filter_pred(matching):
                    remaining += 1
                    left_poss.append(matching)
                    # dump all remaining matches only for debugging purpose
                    if r_out:
                        print_matching_to_file(matching, self.lutA, self.lutB, file=r_out)

        rem = (tuple(each for _ in range(len(self.lutB)*len(self.lutA))),total)
        print_map((tuple(map(lambda x: x/rem[1]*100, rem[0])),rem[1]), len(self.lutB), self.lutA, self.lutB, color)
        print()

        for c in self.constraints:
            rem = c.apply_to_rem(rem)
            c.print_left(self.lutA, self.lutB, color)
            print()

        with open(f"{output_stem}_tab.dot", "w") as dotTab:
            print_map_dot((tuple(map(lambda x: x/rem[1]*100, rem[0])),rem[1]), len(self.lutB), self.lutA, self.lutB, True, dotTab)
        Popen(["dot", "-Tpdf", "-o", f"{output_stem}_tab.pdf", f"{output_stem}_tab.dot"])
        Popen(["dot", "-Tpng", "-o", f"{output_stem}_tab.png", f"{output_stem}_tab.dot"])

        t = PrettyTable()
        t.field_names = [" ", "R", *self.lutA, "", "I"]
        with (open(f"{output_stem}_statMB.out", "w") as mbo,
              open(f"{output_stem}_statMN.out", "w") as mno,
              open(f"{output_stem}_statInfo.out", "w") as info):
            for c in self.constraints:
                c.write_stats(mno, mbo, info)
                t.add_row(c.row(self.lutA, self.lutB))
            print(t.get_string())

        if len(left_poss) <= dot_bound:
            # TODO generate tree
            pass

    @classmethod
    def parse(cls, filename):
        # TODO write error messages
        with open(filename, "r") as file:
            y = yaml.safe_load(file.read())
        assert "setA" in y and isinstance(y["setA"], list), ""
        lutA = y["setA"]
        lutAr = {v:i for i,v in enumerate(lutA)}

        assert "setB" in y and isinstance(y["setB"], list), ""
        lutB = y["setB"]
        lutBr = {v:i for i,v in enumerate(lutB)}

        assert len(lutA) <= len(lutB), ""

        assert "instructions" in y and isinstance(y["instructions"], list), ""
        constraints = [Constraint.parse(x, lutAr, lutBr) for x in y["instructions"]]

        return cls(constraints=constraints, lutA=lutA, lutAr=lutAr, lutB=lutB, lutBr=lutBr)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(formatter_class=argparse.ArgumentDefaultsHelpFormatter)
    parser.add_argument("input", help="path to the input .yaml file")
    parser.add_argument("-c", "--color", help="use color in shell output", default=False, action="store_true")
    parser.add_argument("-d", "--dot-bound", help="dot bound", default=200, type=int)
    parser.add_argument("-o", "--output", help="Output STEM for .dot and .pdf", default="test")
    # if not given, this is None, if given without value this is the const value, if giben with value this is the value
    parser.add_argument("-m", "--matchings", help="Print out the matchings which are left in ordered fashion (especially for debugging)", args='?', const="match.dat")
    # parser.add_argument("-r", "--[no-]reverse", help="use color in shell output", default=False, action="store_true")

    args = parser.parse_args()

    g = Game.parse(filename=args.input)

    print(time.time()) # make output unique
    g.sim(color=args.color, dot_bound=args.dot_bound, output_stem=args.output, print_matchings=args.matchings)
