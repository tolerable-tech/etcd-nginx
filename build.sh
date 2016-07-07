

case $1 in
  "a")
    $0 "build"
    $0 "push"
    ;;
  "push")
    docker push tolerable/nginx_confd
    ;;
  "build")
    version=$(cat VERSION);
    docker build --force-rm -t local/nginx_lb .
    docker tag  local/nginx_lb tolerable/nginx_confd:$version
    ;;
  "test")
    $0 "build"
    docker run -P --rm --name test tolerable/nginx_confd
    ;;
  * )
    $0 "build"
    ;;
esac

