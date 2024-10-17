FROM ubuntu:latest


ARG REPO=ckpool-solo
ARG BRANCH=solobtc
ARG REPO_URL=https://bitbucket.org/ckolivas/${REPO}.git


RUN apt-get update && apt-get install -y autoconf automake libtool build-essential git yasm libzmq3-dev libcap2-bin pkgconf

RUN git clone ${REPO_URL}

WORKDIR /ckpool-solo

COPY conf/ckproxy.conf ./src

RUN ./autogen.sh && ./configure && make

WORKDIR ./src 

CMD ["./ckpool","-p","-k", "-c", "./ckproxy.conf"]
