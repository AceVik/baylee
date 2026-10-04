#!/bin/bash
sed -i '' 's/if let Ok((panel, mut vis)) = panel_query.get_single_mut() {/if let Ok((panel, mut vis)) = panel_query.single_mut() {/g' crates/baylee-client/src/hud/ledge/ai_log.rs
sed -i '' '/TextFont {/,/},/c\
                super::tf(&fonts, 14.0),\
' crates/baylee-client/src/hud/ledge/ai_log.rs
