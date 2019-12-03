FROM rustlang/rust:nightly

WORKDIR /usr/src/regulators
COPY . .

RUN cargo install --path .

CMD ["regulators"]
