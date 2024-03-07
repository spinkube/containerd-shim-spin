FROM busybox
ARG TARGETPLATFORM
COPY script/installer.sh /script/installer.sh
COPY ./.tmp/${TARGETPLATFORM} /assets
CMD sh /script/installer.sh
