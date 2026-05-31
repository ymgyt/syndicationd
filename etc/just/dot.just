import '.base.just'

dot_dir := "etc/dot"
format := "svg"

default: overview authentication

# Generate overview image
[no-cd]
overview:
	@just dot generate overview

[no-cd]
authentication:
	@just dot generate authentication

[no-cd]
[private]
generate src:
	dot -T{{format}} {{justfile_directory()}}/{{dot_dir}}/{{src}}.dot out> {{justfile_directory()}}/{{dot_dir}}/dist/{{src}}.{{format}}
