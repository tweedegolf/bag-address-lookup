FROM ubuntu:26.04 AS final-base
RUN apt-get update && apt-get install adduser -y && apt-get upgrade -y

# create a non root user to run the binary
ARG user=nonroot
ARG group=nonroot
ARG uid=2000
ARG gid=2000
RUN addgroup --gid ${gid} ${group} && adduser --uid ${uid} --gid ${gid} --system --disabled-login --disabled-password ${user}

WORKDIR /home/${user}
USER $user

FROM final-base AS bagatel
ARG version=dev

COPY --chown=nonroot:nonroot ./bagatel-linux-x64 ./bagatel
RUN chmod 755 bagatel

EXPOSE 8080
ENV VERSION=${version}
ENTRYPOINT ["./bagatel"]
CMD [ "0.0.0.0:8080" ]
