

case $1 in
  "a")
    $0 "build"
    $0 "push"
    ;;
  "push")
    docker push bossdjbradley/nginx_confd
    ;;
  "build")
    docker build --force-rm -t local/nginx_lb .
    docker tag --force local/nginx_lb bossdjbradley/nginx_confd
    ;;
  * )
    $0 "build"
    ;;
esac

