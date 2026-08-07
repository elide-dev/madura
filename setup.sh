#!/bin/bash
echo "Setting up madura dev..."
echo "- Adding bin/ to PATH..."
export PATH="$PWD/bin:$PATH"
echo "- Installing toolchains..."
./bin/mise install
echo "- Activating mise..."
eval "$(./bin/mise activate zsh)"
echo "Ready."
