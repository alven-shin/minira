default:
    @just --list

helix:
    hx -v src/main.rs

inspect-log:
    tail -f ~/.cache/helix/helix.log
