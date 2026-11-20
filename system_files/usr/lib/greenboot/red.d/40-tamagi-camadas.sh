#!/usr/bin/bash
# Roda quando a verificação de saúde falha, antes de o computador reiniciar.
# As camadas do tamagi-layer vivem em /var, fora da imagem: um rollback da
# imagem não as tiraria. Então elas são desligadas aqui; se a culpa era de uma
# camada, o próximo boot já passa, sem precisar voltar de imagem.
EXT=/var/lib/extensions
OFF=/var/lib/tamagi/layers-off
compgen -G "$EXT/*.raw" >/dev/null || exit 0
mkdir -p "$OFF"
mv "$EXT"/*.raw "$OFF"/
echo "tamagi: camadas desligadas depois de um boot com falha (religar: tamagi-layer on)"
exit 0
