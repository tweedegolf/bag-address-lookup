FROM ubuntu:24.04 AS final-base
RUN apt-get update && apt-get install adduser -y && apt-get upgrade -y

# create a non root user to run the binary
ARG user=nonroot
ARG group=nonroot
ARG uid=2000
ARG gid=2000
RUN addgroup --gid ${gid} ${group} && adduser --uid ${uid} --gid ${gid} --system --disabled-login --disabled-password ${user}

WORKDIR /home/${user}
USER $user

FROM final-base AS bag-service
ARG version=dev

COPY --chown=nonroot:nonroot ./bag-service-linux-x64 ./bag-service
RUN chmod 755 bag-service

EXPOSE 3000
ENV VERSION=${version}
ENTRYPOINT ["./bag-service"]
CMD [ "0.0.0.0:8080" ]
