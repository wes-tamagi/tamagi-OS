# shellcheck shell=sh
# mise é o gerenciador padrão de linguagens, ferramentas, variáveis e tarefas
# de projeto no Tamagi (https://mise.jdx.dev). Os "shims" entram no PATH de
# toda sessão, inclusive apps gráficos e IDEs; o shell interativo ainda ativa
# o mise por completo (/etc/profile.d/tamagi.sh), que troca de versão por pasta.
if [ -x /usr/bin/mise ]; then
    case ":$PATH:" in
        *":${XDG_DATA_HOME:-$HOME/.local/share}/mise/shims:"*) ;;
        *) PATH="${XDG_DATA_HOME:-$HOME/.local/share}/mise/shims:$PATH"; export PATH ;;
    esac
fi
