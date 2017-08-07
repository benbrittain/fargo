#!/usr/bin/env bash

# Copyright 2017 The Fuchsia Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

set -e

script_dir=$(dirname $BASH_SOURCE)
script_dir=`cd $script_dir; pwd`
target_dir=`cd $script_dir/../target; pwd`
src_dir=$target_dir/src

mkdir -p $src_dir

os_name=`uname`
case $os_name in
Linux)
    N=`cat /proc/cpuinfo | grep processor | wc -l`
    PARALLEL=-j`expr $N + $N`
    ;;
Darwin)
    PARALLEL=-j`sysctl -n hw.ncpu`
    ;;
*)
    PARALLEL=-j8
    ;;
esac

if ! $(fargo pkg-config -- --exists zlib)
then
   echo "Let's install zlib"
   ZLIB_NAME_VERSION=zlib-1.2.11
   ZLIB_ARCHIVE=$ZLIB_NAME_VERSION.tar.gz
   ZLIB_ROOT=$src_dir/$ZLIB_NAME_VERSION

   if [ ! -d "$ZLIB_ROOT" ]; then
       cd $src_dir
       if [ ! -f $ZLIB_ARCHIVE ]; then
           wget http://zlib.net/$ZLIB_ARCHIVE
       fi

       tar xf $ZLIB_ARCHIVE

   fi
   cd $ZLIB_ROOT

   declare -x CHOST="x86_64-fuchsia"
   declare -x LDFLAGS="-Wl,-soname,libz.so.1"

   fargo configure --no-host -- --64 --static
   make $PARALLEL
   make install
fi

if ! $(fargo pkg-config -- --exists libpng)
then
  PNG_NAME_VERSION=libpng-1.6.31
  PNG_ARCHIVE=$PNG_NAME_VERSION.tar.gz
  PNG_ROOT=$src_dir/$PNG_NAME_VERSION

  if [ ! -d "$PNG_ROOT" ]; then
      cd $src_dir
      if [ ! -f $PNG_ARCHIVE ]; then
          wget ftp://ftp-osl.osuosl.org/pub/libpng/src/libpng16/$PNG_ARCHIVE
      fi

      tar xf $PNG_ARCHIVE
  fi

  cd $PNG_ROOT

  fargo configure
  make $PARALLEL
  make install
fi

if ! $(fargo pkg-config -- --exists pixman-1)
then
  PIXMAN_NAME_VERSION=pixman-0.32.6
  PIXMAN_ARCHIVE=$PIXMAN_NAME_VERSION.tar.gz
  PIXMAN_ROOT=$src_dir/$PIXMAN_NAME_VERSION

  if [ ! -d "$PIXMAN_ROOT" ]; then
      cd $src_dir
      if [ ! -f $PIXMAN_ARCHIVE ]; then
          wget https://cairographics.org/releases/$PIXMAN_ARCHIVE
      fi

      tar xf $PIXMAN_ARCHIVE
  fi

  cd $PIXMAN_ROOT

  fargo configure

  case $os_name in
  Linux)
    sed -i '/#define HAVE_GCC_VECTOR_EXTENSIONS.*/d' config.h
    ;;
  Darwin)
    sed -i '' '/#define HAVE_GCC_VECTOR_EXTENSIONS.*/d' config.h
    ;;
  esac

  make $PARALLEL
  make install
fi

if ! $(fargo pkg-config -- --exists freetype2)
then
  FREETYPE_NAME_VERSION=freetype-2.6.5
  FREETYPE_ARCHIVE=$FREETYPE_NAME_VERSION.tar.gz
  FREETYPE_ROOT=$src_dir/$FREETYPE_NAME_VERSION

  if [ ! -d "$FREETYPE_ROOT" ]; then
      cd $src_dir
      if [ ! -f $FREETYPE_ARCHIVE ]; then
          wget http://download.savannah.gnu.org/releases/freetype/$FREETYPE_ARCHIVE
      fi

      tar xf $FREETYPE_ARCHIVE
  fi

  cd $FREETYPE_ROOT

  fargo configure
  make $PARALLEL
  make install
fi

if ! $(fargo pkg-config -- --exists cairo)
then
  CAIRO_NAME_VERSION=cairo-1.14.6
  CAIRO_ARCHIVE=$CAIRO_NAME_VERSION.tar.xz
  CAIRO_ROOT=$src_dir/$CAIRO_NAME_VERSION

  if [ ! -d "$CAIRO_ROOT" ]; then
      cd $src_dir
      if [ ! -f $CAIRO_ARCHIVE ]; then
          wget https://cairographics.org/releases/$CAIRO_ARCHIVE
      fi

      tar xf $CAIRO_ARCHIVE
  fi

  cd $CAIRO_ROOT

  fargo configure
  make $PARALLEL
  make install
fi
