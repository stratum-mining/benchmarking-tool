# FROM ubuntu:latest


# ARG REPO=ckpool-solo
# ARG BRANCH=solobtc
# ARG REPO_URL=https://bitbucket.org/ckolivas/${REPO}.git


# RUN apt-get update && apt-get install -y autoconf automake libtool build-essential git yasm libzmq3-dev libcap2-bin pkgconf

# RUN git clone ${REPO_URL}

# WORKDIR /ckpool-solo

# COPY conf/ckproxy.conf ./src

# RUN ./autogen.sh && ./configure && make

# WORKDIR ./src 

# CMD ["./ckpool","-p","-k", "-c", "./ckproxy.conf"]

FROM ubuntu:latest

RUN apt-get update && apt-get install -y autoconf automake libtool build-essential git yasm libzmq3-dev libcap2-bin pkgconf

ARG REPO_URL=https://github.com/Shourya742/Pool.git

RUN git clone ${REPO_URL}

WORKDIR /Pool

RUN git reset 1a0b3143564d3e45edf8679ff344aac92bff7f25 --hard

COPY conf/ckproxy.conf ./src

RUN ./autogen.sh && ./configure && make

WORKDIR ./src 

CMD ["./ckpool","-p","-k", "-c", "./ckproxy.conf", "-l", "7"]
# CMD ["./ckpool","-B","-k", "-c", "./ckpool.conf", "-l", "7"]
