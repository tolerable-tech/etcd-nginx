
version=$(cat VERSION);

case $1 in
  "a")
    $0 "build"
    $0 "push"
    ;;
  "push")
    docker push tolerable/nginx_confd:$version
    ;;
  "build")
    docker build --force-rm -t local/nginx_lb:$version .
    docker tag  local/nginx_lb:$version tolerable/nginx_confd:$version
    ;;
  "test")
    $0 "build"
    $0 "run"
    ;;
  "run")
    docker run -P --rm --name test tolerable/nginx_confd:$version
    ;;
  "login")
    docker run -P --rm --name test --volume `pwd`:/app -ti tolerable/nginx_confd:$version /bin/bash -l
    ;;
  * )
    $0 "build"
    ;;
esac

