# Cutting a new release of the Containerd Shim Spin

To cut a new release of the `containerd-shim-spin`, you will need to do the
following:

1. Confirm that [CI is
   green](https://github.com/spinkube/containerd-shim-spin/actions) for the commit selected to be
   tagged and released.

1. Change all references of the version number in repository using the `version.sh` script, which
   updates all Cargo manifests and README documentation to use a bumped version. Specify a major,
   minor, or patch version update using the `-m`, `-n`, `-p` flags, respectively.
   ```sh
   # Update minor version (1.2.3 -> 1.3.0)
   version.sh -n
   ```

1. Update [CHANGELOG.md](./CHANGELOG.md). The Unreleased section should be updated with the new
   version number and the date of the release. Update the links to new version tag in the footer of
   CHANGELOG.md.

1. Add a new column to the [README shim and Spin version map](./README.md#shim-and-spin-version-map)
   that lists the version of the Spin dependencies for the release.
   
1. Create a pull request with these changes and merge once approved.

1. Checkout the commit with the version bump from above.

1. Create and push a new tag with a `v` and then the version number.

    As an example, via the `git` CLI:

    ```
    # Create a GPG-signed and annotated tag
    git tag -s -m "Containerd Shim Spin v0.15.0" v0.15.0

    # Push the tag to the remote corresponding to spinkube/containerd-shim-spin (here 'origin')
    git push origin v0.15.0
    ```

1. Pushing the tag upstream will trigger the [release
   action](https://github.com/spinkube/containerd-shim-spin/actions/workflows/release.yaml).
    - The release build will create binary releases of the shim and upload these
      assets to a new GitHub release for the pushed tag. Release notes are
      auto-generated but edit as needed especially around breaking changes or
      other notable items.
    - The release action also creates test applications, a k3d node image with
      the `containerd-shim-spin`, and a new node installer image to be used by
      the runtime class manager.
  
1. Update [SpinKube documentation](https://github.com/spinkube/documentation) as
   necessary. Ensure the latest [node installer
   image](https://www.spinkube.dev/docs/install/installing-with-helm/#prepare-the-cluster)
   is used and update the [Shim and Spin version
   map](https://www.spinkube.dev/docs/reference/shim-spin-version-map/).