#!/bin/bash
sed -i '' -e '190,194c\
        if let Some(hold) = hold { \
            if hold == "until_my_turn" { \
                self.hold_until_my_turn = true; \
            } \
        }
' crates/baylee-seat/src/wake.rs
