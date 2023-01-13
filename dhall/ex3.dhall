

let Config : Type =
      { home : Text
      , privateKey : Text
      , publicKey : Text
      }

let makeUser : Text -> Config = \(user : Text) ->
      let home       : Text   = "/home/${user}"
      let privateKey : Text   = "${home}/.ssh/id_ed25519"
      let publicKey  : Text   = "${privateKey}.pub"
      let config     : Config = { home, privateKey, publicKey }
      in  config

let configs : List Config =
      [ makeUser "bill"
      , makeUser "jane"
      ]

in  configs
