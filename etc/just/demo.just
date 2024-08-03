import '.base.just'

demo_tape := "etc/demo/demo.tape"

# Record demo
record *flags:
    cat etc/demo/vhs_settings.tape out> {{ demo_tape }}
    nix run nixpkgs#vhs -- record --shell nu out>> {{ demo_tape }}

# Run vhs and generate gif
gif:
    nix run nixpkgs#vhs -- {{ demo_tape }}

gif-short:
    nix run nixpkgs#vhs -- etc/demo/demo_short.tape --output etc/demo/demo_short.gif
