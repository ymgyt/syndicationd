import '.base.just'

set working-directory := '../..'

# Record demo.gif on Wayland
rec:
    let wlrec = ($env.WLREC_DIR? | default '../wlrec'); nix develop $wlrec --command nix shell 'nixpkgs#jq' 'nixpkgs#gifsicle' --command bash etc/demo/record.sh $wlrec

# Capture the README and social preview with the current COSMIC wallpaper
ss:
    nix shell 'nixpkgs#sway-unwrapped' 'nixpkgs#swaybg' 'nixpkgs#grim' 'nixpkgs#alacritty' 'nixpkgs#tmux' 'nixpkgs#jq' 'nixpkgs#pngquant' --command bash etc/demo/screenshot.sh
