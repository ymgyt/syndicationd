import '.base.just'

dot_dir := "etc/dot"
format := "svg"

# Generate overview image
[no-cd]
overview:
	dot -T{{format}} {{dot_dir}}/overview.dot out> {{dot_dir}}/overview.{{format}}

