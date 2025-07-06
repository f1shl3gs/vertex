FROM chiseled

COPY vertex /usr/bin/vertex

VOLUME /var/lib/vertex

# ensure vertex can run in this docker image
RUN ["/usr/bin/vertex", "--version"]

ENTRYPOINT ["/usr/bin/vertex"]
