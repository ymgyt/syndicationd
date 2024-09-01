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
[private]
generate src:
	dot -T{{format}} {{dot_dir}}/{{src}}.dot out> {{dot_dir}}/{{src}}.{{format}}
