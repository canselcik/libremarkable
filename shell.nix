with import <nixpkgs> {
  crossSystem = {
    config = "arm-linux-gnueabihf";
  };
};

mkShell {
  buildInputs = [];
}
