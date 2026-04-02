#ifndef EASYSSH_SHIM_H
#define EASYSSH_SHIM_H

/* This shim header provides a stable path for the modulemap.
 * The actual easyssh_core.h is copied here during the build process
 * or can be symlinked from the core/target/include directory.
 */

#include "easyssh_core.h"

#endif /* EASYSSH_SHIM_H */
