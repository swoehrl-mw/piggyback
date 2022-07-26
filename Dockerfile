FROM --platform=$BUILDPLATFORM scratch
ARG TARGETOS
ARG TARGETARCH
COPY multiarch/${TARGETOS}/${TARGETARCH} /piggyback-proxy
CMD ["/piggyback-proxy"]
