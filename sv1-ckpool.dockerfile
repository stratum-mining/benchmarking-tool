# syntax=docker/dockerfile:1
FROM debian:11-slim

ARG REPO=ckpool-solo
ARG BRANCH=solobtc
ARG REPO_URL=https://bitbucket.org/ckolivas/${REPO}.git


ENV BUILD_DIR=/var/build
ENV BIN_DIR=/srv/ckpool

ENV USER_UID=1000
ENV USER_GID=1000
ENV USER_NAME=ckpool

# create ckpool group and user
RUN groupadd --gid ${USER_GID} ${USER_NAME} \
    && useradd --uid ${USER_UID} --gid ${USER_GID} -m ${USER_NAME}

# install required packages
RUN apt-get update && apt-get install -y autoconf automake libtool build-essential git yasm libzmq3-dev libcap2-bin

# fetch sources from github
WORKDIR ${BUILD_DIR}
RUN git clone ${REPO_URL}

# build ckpool-solo sources
WORKDIR ${BUILD_DIR}/${REPO}
RUN git checkout ${BRANCH}
RUN ./autogen.sh && ./configure --prefix=${BIN_DIR}
RUN make

# install binaries
RUN make install

# setup conf and logs directories
RUN mkdir -p ${BIN_DIR}/conf
COPY conf/ckpool.conf ${BIN_DIR}/conf
RUN mkdir -p ${BIN_DIR}/logs
RUN chown -R ${USER_NAME}:${USER_NAME} ${BIN_DIR}

# final configuration
EXPOSE 3333
WORKDIR ${BIN_DIR}

# switch to ckpool user
USER ${USER_NAME}

# start ckpool
CMD rm -f /tmp/ckpool/main.pid \
    && ./bin/ckpool -B -c ./conf/ckpool.conf