let makeUser = \(user : Text) ->
      let home  : Text     = "/home/${user}"
      let privateKey = "${home}/.ssh/id_ed25519"
      let publicKey  = "${privateKey}.pub"
      in  { home, privateKey, publicKey }

in  [ makeUser "bill"
    , makeUser "jane"
    ]
