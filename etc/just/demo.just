import '.base.just'

demo_tape := "etc/demo/demo.tape"

# Record demo
[no-cd]
record *flags:
    cat etc/demo/vhs_settings.tape out> {{ demo_tape }}
    nix run nixpkgs#vhs -- record --shell nu out>> {{ demo_tape }}

# Run vhs and generate gif
[no-cd]
gif:
    nix run nixpkgs#vhs -- {{ demo_tape }}

[no-cd]
gif-short:
    nix run nixpkgs#vhs -- etc/demo/demo_short.tape --output etc/demo/demo_short.gif

# Record demo with t-rec
[no-cd]
trec *flags:
    TERM_PROGRAM=alacritty nix run nixpkgs#t-rec -- --output etc/demo/demo
