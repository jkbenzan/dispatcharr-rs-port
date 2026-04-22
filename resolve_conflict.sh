#!/bin/bash
sed -i '232,319d' src/api.rs
sed -i 's/>>>>>>> origin\/main//g' src/api.rs
