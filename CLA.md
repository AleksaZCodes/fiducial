# Contributor License Agreement

**This is not legal advice, and it has not been reviewed by counsel.** It is
drafted to satisfy [`IP-POLICY.md`](IP-POLICY.md) rule 3 and should be reviewed
by an attorney before the first outside contribution is merged. The same
disclaimer that opens `IP-POLICY.md` applies to this file.

---

## Why this exists before it is needed

`IP-POLICY.md` rule 3:

> **CLA before accepting any outside contribution.** Without a Contributor
> License Agreement, dual-licensing the project later becomes impossible. No
> external contribution is merged before a CLA is in place.

This repository is public and can accept pull requests today. A contribution
merged without a CLA is owned by the person who wrote it, and every future
licensing decision then needs that person's permission — individually, for
every contributor, forever. People change email addresses, lose interest, and
die. There is no mechanism to fix it afterwards.

Zero contributions is the cheapest this will ever be, and the cost rises by one
person each time it is deferred. That is the whole argument: not that
relicensing is planned, but that the option is destroyed silently and cannot be
recovered.

## Agreement

By submitting a contribution to this project, You accept and agree to the
following terms for Your present and future contributions. Except for the
licenses granted here, You retain all right, title and interest in Your
contributions.

### 1 · Definitions

**"You"** means the copyright owner, or the legal entity authorised by the
copyright owner, entering into this agreement.

**"Contribution"** means any work of authorship submitted by You to this
project — code, documentation, configuration, hardware description, or any
other material — in any form, including but not limited to a pull request, a
patch, or an issue comment containing material intended for inclusion.

**"Project Owner"** means Aleksa Zdravkovic, the copyright holder named in
[`LICENSE`](LICENSE).

### 2 · Copyright license

You grant the Project Owner a perpetual, worldwide, non-exclusive, no-charge,
royalty-free, irrevocable copyright license to reproduce, prepare derivative
works of, publicly display, publicly perform, sublicense and distribute Your
Contribution and such derivative works.

**This license includes the right to sublicense and to distribute Your
Contribution under any license terms, including licenses other than the license
this project currently uses.** This sentence is the operative one. It is what
preserves the ability to dual-license, and it is the reason this document
exists rather than relying on the inbound-equals-outbound convention.

### 3 · Patent license

You grant the Project Owner a perpetual, worldwide, non-exclusive, no-charge,
royalty-free, irrevocable patent license to make, have made, use, offer to
sell, sell, import and otherwise transfer Your Contribution, where such license
applies only to those patent claims licensable by You that are necessarily
infringed by Your Contribution alone or by combination of Your Contribution
with the project to which it was submitted.

If any entity institutes patent litigation against You or any other entity
alleging that Your Contribution, or the project it was submitted to,
constitutes direct or contributory patent infringement, then any patent
licenses granted to that entity under this agreement terminate as of the date
such litigation is filed.

Note that this grant runs **to the Project Owner**, not to the public.
`IP-POLICY.md` rule 2 records why the project uses MIT rather than Apache-2.0:
Apache-2.0 contains an express patent grant to all recipients, and retaining
patent rights matters here. A CLA that granted patent rights to the world would
undo that choice through the back door.

### 4 · Representations

You represent that:

1. Each Contribution is Your original creation, or You have the right to submit
   it under this agreement.
2. If Your employer has rights to intellectual property You create, You have
   received permission to make the Contribution on behalf of that employer, or
   Your employer has waived such rights.
3. Your Contribution does not knowingly include third-party material subject to
   terms incompatible with this agreement. Where a Contribution includes any
   third-party material, You identify it and its license in the pull request.
4. You are not knowingly submitting material subject to a patent, trade secret,
   or confidentiality obligation that would be violated by its publication.

### 5 · No obligation, and no warranty

The Project Owner is under no obligation to accept, merge, or use any
Contribution. Except for the representations in section 4, You provide Your
Contribution **"AS IS", without warranty of any kind**, express or implied,
including without limitation any warranty of merchantability, fitness for a
particular purpose, or non-infringement.

### 6 · Notice of change

If You become aware that any representation in section 4 is or has become
inaccurate, You agree to notify the Project Owner.

---

## How to sign

Add a `Signed-off-by` trailer to every commit in Your pull request:

```sh
git commit -s -m "your message"
```

which appends:

```
Signed-off-by: Your Name <your.email@example.com>
```

That trailer certifies that You have read this agreement and that You agree to
it for that contribution. The name and email must be real and must match the
commit author.

A sign-off is used rather than a signature service because the trailer lives in
git history, next to the contribution it covers, and survives any change of
forge or tooling. A record of consent stored somewhere other than the artifact
it consents to is a second declaration that can drift from it — which is the
failure mode this whole project is organised against.

## Scope

This agreement covers the **public tier** only: the platform, CLI, guardrails,
templates and protocol primitives published from this repository.
`IP-POLICY.md` defines a private tier — product repositories, domain logic,
novel protocols and algorithms, and hardware designs — which is never published
and never accepts outside contributions, so nothing here applies to it.
