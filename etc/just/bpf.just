# Build ebpf program
[no-cd]
build *flags:
    nix develop .#ebpf --command cargo task build-ebpf {{ flags }}
