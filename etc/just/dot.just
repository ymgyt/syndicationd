import '.base.just'

dot_dir := "etc/dot"
format := "svg"

default: overview packages

# Generate overview image
[no-cd]
overview:
	@just dot generate overview

# Generate packages image
[no-cd]
packages:
	@just dot generate packages

[no-cd]
authentication:
	@just dot generate authentication

[no-cd]
kvsd:
	@just dot generate kvsd_architecture

[no-cd]
[private]
generate src:
	dot -T{{format}} {{dot_dir}}/{{src}}.dot out> {{dot_dir}}/dist/{{src}}.{{format}}
