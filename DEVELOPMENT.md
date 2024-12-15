# Development manual

## Tenets

When struggling with a difficult technical decision, refer to these. Add another one if the existing tenets didn't help.

1. **Have fun**. This is a hobby project not constrained by monetary returns or timelines.
2. **Prioritize your use-case**. There is a lot of variation in hardware and OS versions. Make sure the software works 
for _you_ and provide a reasonable fallback for the others instead of getting stuck on 100% implementation.

## Logging

### Where to log?

You are encouraged to add log statements to **all** "interesting" points of your code. The following situations are
considered "interesting":
* The app encounters an **unexpected** situation (e.g. a library call that is expected to succeed fails).
Regardless of whether it leads to a crash or the app can recover, log it as `error!`.
* The app encounters an **undesired**, but expected situation (e.g. newly opened window can't receive focus due to a
context menu being displaying by another application). Log it as `warn!`.
* Major milestones of the **app lifecycle** (e.g. app has started or app will shut down). Log it as `info!`.
* The app enters context of a **known or hypothetical bug** that the developers would like to learn more about.
Log it as `debug!`.
* The app enters a branch of code that **alters app state** (e.g. a setting change led to UI rebuild).
Log it as `trace!`.

### What to log?

* The message should clearly explain what happened (or what is about to happen).
* Add relevant context (such as error code).
* Don't add information that can be deduced from the context.
* Don't add variable values unless these values are crucial to understanding why something happened.