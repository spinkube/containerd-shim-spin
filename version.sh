#!/bin/bash

OTHER_FILES=${MD_FILES_TO_UPDATE:-"containerd-shim-spin/quickstart.md, images/spin-dapr/README.md, deployments/k3d/README.md"}

get_version()
{
    pkg_version=$(cargo pkgid --package containerd-shim-spin-v2)
    version=$(echo $pkg_version | sed -E 's/.*@([0-9]+\.[0-9]+\.[0-9]+)$/\1/')
    echo $version
}

new_version()
{
    OLD_VERSION=$1
    if [ "$SAME" != "1" ]; then
        if [ "$MAJOR" == "1" ]; then
            NEW_VERSION="$( echo $OLD_VERSION | awk -F '.' '{print $1 + 1}' ).0.0"
        elif [ "$MINOR" == "1" ]; then
            NEW_VERSION="$( echo $OLD_VERSION | awk -F '.' '{print $1}' ).$( echo $OLD_VERSION | awk -F '.' '{print $2 + 1}' ).0"
        elif [ "$PATCH" == "1" ]; then
            NEW_VERSION="$( echo $OLD_VERSION | awk -F '.' '{print $1}' ).$( echo $OLD_VERSION | awk -F '.' '{print $2}' ).$( echo $OLD_VERSION | awk -F '.' '{print $3 + 1}' )"
        fi
    else
        NEW_VERSION=$OLD_VERSION
    fi
    echo "$NEW_VERSION"
}

update_file_version()
{
    FILE=$1
    LINE_PATTERN=$2
    NEW_LINE=$3

    echo "Update $FILE [$LINE_PATTERN] => [$NEW_LINE]"
    sed -i s/"$LINE_PATTERN"/"$NEW_LINE"/g $FILE
    return $?
}


BASEDIR=$(dirname "$0")

SAME=0
MAJOR=0
MINOR=0
PATCH=1


while getopts mnphs option
do
case "${option}" in
m) MAJOR=1;;
n) MINOR=1;;
p) PATCH=1;;
s) SAME=1;;
h)
    echo "Increments versions depending on the semver option chosen"
    echo "Usage: version.sh [-u] [-c]"
    echo "    -s updates all files to reflect the current base Cargo.toml version"
    echo "    -m increments major version and sets minor and patch to 0"
    echo "    -n increments minor version and sets patch to 0"
    echo "    -p increments patch version"
    exit 0
    ;;
*)
    echo "Invalid option: -$OPTARG" >&2
    echo "Usage: version.sh [-s] [-m] [-n] [-p] [-h]"
    exit 1
    ;;
esac
done

OLD_VERSION=$(get_version)
NEW_VERSION=$(new_version $OLD_VERSION)

echo "Updating to version: $NEW_VERSION"

TOML_VERSION_PATTERN="^version = .*"
TOML_VERSION_LINE="version = \"$NEW_VERSION\""

# 1) Update base Cargo.toml and workspace lockfile
update_file_version "$BASEDIR/Cargo.toml" "$TOML_VERSION_PATTERN" "$TOML_VERSION_LINE"
if [ "$?" -eq "1" ]; then exit 1; fi
cargo update

# 2) Update test images and lockfiles
for file in $(find $BASEDIR/images -name Cargo.toml); do
    update_file_version "$file" "$TOML_VERSION_PATTERN" "$TOML_VERSION_LINE"
    if [ "$?" -eq "1" ]; then exit 1; fi
    dir=$(dirname $file)
    if [[ $dir != /* ]]; then
        dir=$BASEDIR/$dir
    fi
    cargo update --manifest-path $dir/Cargo.toml
done

# 3) Update all requested markdown file versions to $NEW_VERSION
for file in $(echo $OTHER_FILES | tr ',' ' '); do
    update_file_version "$BASEDIR/$file" $OLD_VERSION $NEW_VERSION
    if [ "$?" -eq "1" ]; then exit 1; fi
done

exit 0