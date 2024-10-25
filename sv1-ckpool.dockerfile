FROM ubuntu:latest

RUN apt-get update && apt-get install -y autoconf automake libtool build-essential git yasm libzmq3-dev libcap2-bin pkgconf

ARG REPO_URL=https://github.com/Shourya742/Pool.git

RUN git clone ${REPO_URL}

WORKDIR /Pool

COPY conf/ckpool.conf ./src

RUN ./autogen.sh && ./configure && make

WORKDIR ./src 

CMD ["./ckpool","-B","-k", "-c", "./ckpool.conf", "-l", "7"]
