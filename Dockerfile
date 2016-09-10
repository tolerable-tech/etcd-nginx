FROM nginx

MAINTAINER Jake Wilkins <me at jsw dot io>

EXPOSE 80
EXPOSE 443

RUN apt-get -qq update && apt-get install -qqy curl netcat vim-tiny

RUN curl -L https://raw.githubusercontent.com/Neilpang/acme.sh/master/acme.sh -o ./le.sh && mv ./le.sh /usr/local/bin/le.sh && chmod +x /usr/local/bin/le.sh

RUN apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* 

VOLUME /etc/acme

ADD nginx /etc/nginx
RUN /bin/rm /etc/nginx/conf.d/default.conf
ADD le-auth.conf /etc/nginx/conf.d/le-auth.off
ADD domain-validation.conf /etc/nginx/conf.d/domain-validation.conf
VOLUME /etc/nginx

ADD /le-fetch /usr/local/bin/le-fetch
ADD target/x86_64-unknown-linux-musl/release/conf_manager /usr/local/bin/conf_manager

CMD ["/usr/local/bin/conf_manager"]

RUN mkdir -p /srv/levalidate /srv/fulcrum /srv/sticky
ADD ssl_placeholder.html /srv/fulcrum/ssl_placeholder.html

# moved this down because it changes most often.
ADD ssl.mustache /usr/local/lib/conf_templates/ssl.mustache
ADD ssl_apps.mustache /usr/local/lib/conf_templates/ssl_apps.mustache
ADD non_ssl_apps.mustache /usr/local/lib/conf_templates/non_ssl_apps.mustache
ADD ssl_static_folders.mustache /usr/local/lib/conf_templates/ssl_static_folders.mustache
ADD static_folders.mustache /usr/local/lib/conf_templates/static_folders.mustache
#ADD le.tmpl /etc/confd/templates/le.tmpl
# This has to come after ADD for the Add to persist?
VOLUME /usr/local/lib/conf_templates

