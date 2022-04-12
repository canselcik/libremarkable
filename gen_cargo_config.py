#!/usr/bin/env python3

import os
import re

cargo_config = """
[target.armv7-unknown-linux-gnueabihf]
linker = "{gcc_path}"
rustflags = [
  "-C", "link-arg=-march=armv7-a",
  "-C", "link-arg=-marm",
  "-C", "link-arg=-mfpu=neon",
  "-C", "link-arg=-mfloat-abi=hard",
  "-C", "link-arg=-mcpu=cortex-a9",
  "-C", "link-arg=--sysroot={sysroot}",
]\n"""

def main():
    #get sysroot from environment
    sysroot = os.environ["SDKTARGETSYSROOT"]
    #get comiler name from environment
    cc = os.environ["CC"].split(" ")[0]
    #strip -gcc from compiler to build regex string for parsing PATH
    cc_path = cc[0:-4]
    re_parse_path = ":[^:]*{cc}:".format(cc=cc_path)
    extract_compiler_path = re.compile(re_parse_path)
    toolchain_compiler_path = extract_compiler_path.search(os.environ["PATH"]).group(0)[1:-1]
    #join path to get full compiler path
    cc = os.path.join(toolchain_compiler_path, cc)
    
    #create .cargo dir if not exsits
    if not os.path.isdir(".cargo"):
        os.mkdir(".cargo")
    with open(".cargo/config","w") as f:
        f.write(cargo_config.format(gcc_path=cc, sysroot=sysroot))
    print("Toolchain config written to \".cargo/config\" ✅")

if __name__ == "__main__":
    main()
