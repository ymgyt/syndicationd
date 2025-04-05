import '.base.just'

alias update := update-advisory-db

# Run audit
[no-cd]
default:
    nix build .#checks.{{arch()}}-{{os}}.audit --print-build-logs 

# Update advisory db
[no-cd]
update-advisory-db:
    nix flake update advisory-db
