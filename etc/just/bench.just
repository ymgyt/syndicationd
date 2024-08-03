import '.base.just'

# Run benchmark
[no-cd]
default:
	cargo bench \
		--package synd-term \
		--bench render \
		--features integration \
		-- --verbose
	@start {{justfile_dir()}}/target/criterion/report/index.html

# Generate flamegraph
flamegraph:
	cargo flamegraph \
		--root --output target/flamegraph.svg \
		--package synd-term --bench render --features integration \
		-- --bench
	@start {{justfile_dir()}}/target/flamegraph.svg
