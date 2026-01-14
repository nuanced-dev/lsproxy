
- Release automation with scripts/release-images.sh
- Build images with scripts/build-images.sh -> requires containerd storage for Docker
- Publish images with scripts/publish-images.sh -> prefer to use release!
- All scripts now use similar flags, all support --help
- Docker tests are behind a `docker-tests` feature because they require images to be present
- Terminology: service, image, multi-platform
- Docker files require image version and container registry build args. Just to make sure they are built with the right values.
- Add release automation
