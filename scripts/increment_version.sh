#!/bin/bash

set -e

increment_version() {
  major=${1%%.*}
  minor=$(echo ${1#*.} | sed -e "s/\.[0-9]*//")
  revision=${1##*.}
  echo ${major}.${minor}.$((revision+1))
}

VERSION=$(cat Cargo.toml | grep "version = " | head -n 1 | sed -e 's/version = "\(.*\)"/\1/')

argument_version=$1
if [ -z "$argument_version" ]
then
  NEW_VERSION=$(increment_version ${VERSION})
else
  NEW_VERSION=$1
fi

if ! [[ "${NEW_VERSION}" =~ ^[0-9]\.[0-9].[0-9]*$ ]]
then
  echo "New version is invalid: ${NEW_VERSION}"
  exit 1
fi

echo "Last version was ${VERSION}"
echo "New version is ${NEW_VERSION}"

set -x

sed -i -e "s/^version = \"${VERSION}\"$/version = \"${NEW_VERSION}\"/" Cargo.toml

# Update changelog
sed -i -e "3{
/##/b
s/^/## ${NEW_VERSION}\n\n/
}" CHANGELOG.md
