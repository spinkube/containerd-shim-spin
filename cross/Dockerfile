ARG CROSS_BASE_IMAGE
FROM $CROSS_BASE_IMAGE

# Install Alpine's `apk` to be able to install musl packages
COPY --from=jorgeprendes420/apk-anywhere / /
ENV MARCH=${CROSS_CMAKE_SYSTEM_PROCESSOR}
RUN apk-init ${MARCH} ${CROSS_SYSROOT}

# configure libsecccomp-rs to use static linking
RUN apk-${MARCH} add libseccomp-static libseccomp-dev
ENV LIBSECCOMP_LINK_TYPE="static"
ENV LIBSECCOMP_LIB_PATH="${CROSS_SYSROOT}/lib"

# wws needs zlib (though libssh2-sys)
RUN apk-${MARCH} add zlib-dev zlib-static

# See https://github.com/fermyon/spin/issues/1786 for the upstream issue requiring this polyfill.
RUN --mount=type=bind,from=jorgeprendes420/gcc_vld1q_s8_x4_polyfill,source=/polyfill.sh,target=/polyfill.sh /polyfill.sh

## as per analysis done in https://github.com/fermyon/spin/pull/2287#issuecomment-1970145410 we need
## to disable the cmake config in cross-rs. Setting CROSS_SYSROOT=/ seems to have done the trick
ENV CROSS_SYSROOT=/

RUN apt-get -y update && \
    apt-get install -y pkg-config protobuf-compiler
