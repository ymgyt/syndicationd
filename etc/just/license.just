import '.base.just'

license_file := 'THIRDPARTY.toml'

# Generate dependencies licenses
[no-cd]
default: 
    cargo bundle-licenses --format toml --output {{ license_file }}

# Check dependencies licenses
[no-cd]
check:
    try { RUST_LOG=error cargo bundle-licenses --format toml --output __CHECK --previous {{ license_file }} --check-previous }
    rm __CHECK
