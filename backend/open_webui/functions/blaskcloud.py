"""
title: BlaskCloud - Fallback Orchestrator
description: Prova i modelli free in ordine finché uno risponde. Mai un euro per modelli cloud.
author: BlaskUI
version: 1.0.0
license: MIT
"""

import json
import os
import httpx
from typing import Optional
from pydantic import BaseModel


class Pipe:
    class Valves(BaseModel):
        # Modelli free da provare in ordine (fallback chain)
        FALLBACK_CHAIN: str = "openai/gpt-4o-mini,anthropic/claude-3-haiku,google/gemini-pro"
        # Timeout per ogni modello (secondi)
        TIMEOUT: int = 30
        # API key per i modelli cloud
        API_KEY: str = ""
        # Base URL per le API cloud
        BASE_URL: str = "https://openrouter.ai/api/v1"

    def __init__(self):
        self.type = "pipe"
        self.id = "blaskcloud"
        self.name = "BlaskCloud"
        self.valves = self.Valves()

    async def pipe(self, body: dict, __user__: dict, __metadata__: dict) -> dict:
        """
        Pipe che implementa fallback automatico tra modelli free.
        Prova ogni modello nella FALLBACK_CHAIN finché uno risponde.
        """
        chain = [m.strip() for m in self.valves.FALLBACK_CHAIN.split(",") if m.strip()]
        if not chain:
            return body

        # Prepara il body per il primo modello
        current_body = body.copy()
        current_body["model"] = chain[0]

        # Prova ogni modello nella chain
        for i, model_id in enumerate(chain):
            current_body["model"] = model_id

            try:
                # Usa l'API interna di Open WebUI per fare la chiamata
                response = await self._try_model(current_body, model_id)
                if response:
                    # Aggiungi metadata sul modello usato
                    if "__metadata__" not in current_body:
                        current_body["__metadata__"] = {}
                    current_body["__metadata__"]["blaskcloud_used_model"] = model_id
                    return current_body
            except Exception as e:
                # Log e continua al prossimo modello
                print(f"[BlaskCloud] {model_id} fallito: {e}")
                continue

        # Se tutti falliscono, ritorna il body originale con il primo modello
        current_body["model"] = chain[0]
        return current_body

    async def _try_model(self, body: dict, model_id: str) -> Optional[dict]:
        """
        Prova a fare una chiamata al modello usando l'API OpenAI-compatible.
        Ritorna True se il modello risponde, False altrimenti.
        """
        try:
            api_key = self.valves.API_KEY or os.getenv("OPENAI_API_KEY", "")
            if not api_key:
                return None

            headers = {
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json",
            }

            async with httpx.AsyncClient(timeout=self.valves.TIMEOUT) as client:
                response = await client.post(
                    f"{self.valves.BASE_URL}/chat/completions",
                    headers=headers,
                    json=body,
                )
                if response.status_code == 200:
                    return response.json()
                return None
        except Exception:
            return None
