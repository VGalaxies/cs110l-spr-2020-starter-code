#/bin/bash
rm -f in
for i in $(seq 1 $1)
do
  echo $RANDOM >> in
done
