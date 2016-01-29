FROM nginx

MAINTAINER Jake Wilkins <me at jsw dot io>

EXPOSE 80
EXPOSE 443

RUN apt-get -qq update && apt-get install -qqy curl netcat vim-tiny

RUN curl -L https://github.com/kelseyhightower/confd/releases/download/v0.10.0/confd-0.10.0-linux-amd64 -o confd \
  && mv confd /usr/local/bin/confd && chmod +x /usr/local/bin/confd

RUN curl -L https://raw.githubusercontent.com/Neilpang/le/master/le.sh -o ./le.sh && mv ./le.sh /usr/local/bin/le.sh && chmod +x /usr/local/bin/le.sh

RUN apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* 

ADD nginx-confd.toml /etc/confd/conf.d/nginx.toml
ADD le-confd.toml /etc/confd/conf.d/le.toml

VOLUME /etc/acme
# Nginx tuning
#RUN rm /etc/nginx/conf.d/default.conf
#RUN sed -i '/access_log/a\
    #log_format upstreamlog "[$time_local] $remote_addr passed to: $upstream_addr: $request Upstream Response Time: $upstream_response_time Request time: $request_time";' /etc/nginx/nginx.conf
ADD nginx /etc/nginx
VOLUME /etc/nginx

# add confd-watch script
ADD /confd-watch /usr/local/bin/confd-watch
ADD /le-confd-watch /usr/local/bin/le-confd-watch
ADD /le-domain-check /usr/local/bin/le-domain-check

CMD ["/usr/local/bin/confd-watch"]

RUN mkdir -p /srv/levalidate

# moved this down because it changes most often.
ADD nginx-conf-templ /etc/confd/templates/nginx.tmpl
ADD le.tmpl /etc/confd/templates/le.tmpl
# This has to come after ADD for the Add to persist?
VOLUME /etc/confd
