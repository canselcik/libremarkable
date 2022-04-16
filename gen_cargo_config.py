#!/usr/bin/env python3

import os
import re

cargo_config = """
[target.armv7-unknown-linux-gnueabihf]
linker = "{gcc_path}"
rustflags = [
  "-C", "link-arg=-march=armv7-a",
  "-C", "link-arg=-marm",
  "-C", "link-arg={fpu}",
  "-C", "link-arg={float_s}",
  "-C", "link-arg={cpu}",
  "-C", "link-arg=--sysroot={sysroot}",
]\n"""

def main():
    #get sysroot from environment
    if "SDKTARGETSYSROOT" not in os.environ:
        print("No cross compilation environment found!")
        print("Please set one up by sourcing a environment file for a chosen toolchain.")
        print("Examples for well known toolchains in default locations:")
        print("Codex-rm11x: $ source /opt/codex/rm11x/3.1.15/environment-setup-cortexa7hf-neon-remarkable-linux-gnueabi")
        print("Codex-rm10x: $ source /opt/codex/rm10x/3.1.15/environment-setup-cortexa9hf-neon-remarkable-linux-gnueabi")
        print("Oecore: $ source /usr/local/oecore-x86_64/environment-setup-cortexa9hf-neon-oe-linux-gnueabi")
        #print("Poky: $ source /opt/poky/2.1.3/environment-setup-cortexa9hf-neon-poky-linux-gnueabi")
        exit(1)
    sysroot = os.environ["SDKTARGETSYSROOT"]
    #get comiler name from environment
    cc_full = os.environ["CC"]
    cc = cc_full.split(" ")[0]
    #strip -gcc from compiler to build regex string for parsing PATH
    cc_path = cc[0:-4]
    re_parse_path = ":[^:]*{cc}:".format(cc=cc_path)
    extract_compiler_path = re.compile(re_parse_path)
    toolchain_compiler_path = extract_compiler_path.search(os.environ["PATH"]).group(0)[1:-1]
    #get other arguments from compiler definition
    re_fpu = re.compile("(-mfpu=[^ ]*)")
    re_float = re.compile("(-mfloat-abi=[^ ]*)")
    re_cpu = re.compile("(-mcpu=[^ ]*)")
    fpu_arg = re_fpu.search(cc_full).group(0)
    float_arg = re_float.search(cc_full).group(0)
    cpu_arg = re_cpu.search(cc_full).group(0)
    #join path to get full compiler path
    cc = os.path.join(toolchain_compiler_path, cc)
    
    #create .cargo dir if not exsits
    if not os.path.isdir(".cargo"):
        os.mkdir(".cargo")
    with open(".cargo/config","w") as f:
        f.write(cargo_config.format(gcc_path=cc, sysroot=sysroot, fpu=fpu_arg, float_s=float_arg, cpu=cpu_arg))
    print("Toolchain config written to \".cargo/config\" ✅")

if __name__ == "__main__":
    main()
