# This script takes care of building your crate and packaging it for release

set -ex

main() {
    local src=$(pwd) \
          stage=

    case $TRAVIS_OS_NAME in
        linux)
            stage=$(mktemp -d)
            ;;
        osx)
            stage=$(mktemp -d -t tmp)
            ;;
    esac

    test -f Cargo.lock || cargo generate-lockfile

    cross rustc --bin rs-collector --target $TARGET --release -- -C lto

    cp target/$TARGET/release/rs-collector $stage/

    cd $stage
    tar czf $src/$CRATE_NAME-$TRAVIS_TAG-$TARGET.tar.gz *

    cd $src/distribution/deb
    SRC_DIR=$src BIN=$src/target/$TARGET/release/rs-collector ARCH=$DEPLOY_ARCH VERSION=$TRAVIS_TAG TAG=$TRAVIS_TAG DIST=trusty make package

    cd $src
    rm -rf $stage
}

main
