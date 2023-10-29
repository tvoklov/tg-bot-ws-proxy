# telegram bot websocket proxy

i don't like that telegram requires that you use a webhook for bots. i much prefer discord's approach of using
websockets.

which is why i made this load-balancer-lookalike that you can put on some free tier machine and use as a websocket proxy
between you and your telegram bot.

### tl;dr all it does is:

1. updates telegram's webhook to the ip of whatever machine you've set it to, as well as giving it your certs (
   specifically for people [that use self-signed](https://core.telegram.org/bots/webhooks#a-self-signed-certificate))
2. caches every message that it gets from telegram
3. forwards these messages to any machine that's connected to it via websockets

this could've been done with a kafka connect driver, but i wanted to learn rust

### how to configure:

i like config files, but rust is a headache to me so for now you'll have to settle with configuring in the rust sources
and recompiling the entire project.

the application's config is initiated at [src/main.rs:36](src/main.rs?plain=L36). it's [kinda
straight-forward](https://core.telegram.org/bots/api#setwebhook), which
means no further documentation is necessary.

### the bad:

1. i don't know rust. the code sucks. like, really, i read the rust book about a year ago and tried my best to figure
   things out, but i've probably still made a ton of bad decisions (like using `expect` anywhere where i couldn't be
   bothered to deal with `Result` values)
2. i manage webhook disconnects via trying to use them and, when failing, going "oh well, didn't work, guess that
   webhook is gone now!"
3. currently only one concurrent webhook connection is supported (soft limit, implementation-wise made by literally just
   sending updates to the first webhook in the list). i wanted to add a simple round-robin algorithm, but it's 2 am and
   i wanted to check out a new minecraft modpack. might add this later.
4. i tried my best to figure out the local ip address with rust to make the "set your ip in config" step optional, but
   in the end I guess it's better if you set it up yourself. it's a feature now.
5. i only tested this on windows.