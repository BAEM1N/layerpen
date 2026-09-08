"""Provider wire formats. No credentials are persisted or included in events."""
import base64
from urllib.parse import urlencode

DEFAULT_MODELS = {
    'openai': 'gpt-live-transcribe',
    'gemini': 'gemini-3.5-transcribe-live',
    'elevenlabs': 'scribe_v2_realtime',
    'whisper': 'base',
    'qwen': 'Qwen/Qwen3-ASR-0.6B',
}

class SttError(RuntimeError):
    """Safe, application-authored error, without provider response bodies."""


class Protocol:
    def __init__(self, config):
        self.provider = config['provider']
        self.model = config.get('model') or DEFAULT_MODELS[self.provider]
        self.language = config.get('language', 'auto')
        self.key = config.get('api_key', '')
        self.partials = {}
        self.sequence = 0
        self.order = {}
        self.latest = -1
        self.rate = 24000 if self.provider == 'openai' else 16000

    def connection(self):
        if not self.key:
            raise SttError('API key is required. Enter it in the caption window.')
        if self.provider == 'openai':
            return 'wss://api.openai.com/v1/realtime?intent=transcription', {'Authorization': 'Bearer ' + self.key}
        if self.provider == 'gemini':
            return ('wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent?' + urlencode({'key': self.key}), {})
        query = dict(model_id=self.model, audio_format='pcm_16000', commit_strategy='vad')
        if self.language != 'auto':
            query['language_code'] = self.language
        return 'wss://api.elevenlabs.io/v1/speech-to-text/realtime?' + urlencode(query), {'xi-api-key': self.key}

    def setup(self):
        if self.provider == 'openai':
            transcription = {'model': self.model}
            if self.language != 'auto':
                transcription['languages' if self.model in ('gpt-live-transcribe', 'gpt-transcribe') else 'language'] = [self.language] if self.model in ('gpt-live-transcribe', 'gpt-transcribe') else self.language
            return {'type': 'session.update', 'session': {'type': 'transcription', 'audio': {'input': {
                'format': {'type': 'audio/pcm', 'rate': self.rate},
                'transcription': transcription,
                'turn_detection': {'type': 'server_vad', 'silence_duration_ms': 600, 'prefix_padding_ms': 300},
            }}}}
        if self.provider == 'gemini':
            language = {'ko': 'ko-KR', 'en': 'en-US', 'ja': 'ja-JP', 'zh': 'cmn-Hans-CN'}.get(self.language, self.language)
            return {'setup': {'model': 'models/' + self.model.removeprefix('models/'),
                'generationConfig': {'responseModalities': ['TEXT']},
                'inputAudioTranscription': {'languageCodes': [] if self.language == 'auto' else [language]}}}
        return None

    def audio(self, pcm):
        encoded = base64.b64encode(pcm).decode('ascii')
        if self.provider == 'openai':
            return {'type': 'input_audio_buffer.append', 'audio': encoded}
        if self.provider == 'gemini':
            return {'realtimeInput': {'audio': {'data': encoded, 'mimeType': 'audio/pcm;rate=16000'}}}
        return {'message_type': 'input_audio_chunk', 'audio_base_64': encoded, 'sample_rate': 16000}

    def receive(self, event):
        kind = event.get('type', event.get('message_type', ''))
        if event.get('error') or kind.endswith('failed') or kind in ('auth_error', 'rate_limited', 'quota_exceeded', 'error'):
            # Provider errors may echo credentials/URLs. Expose only an allowlisted category.
            raise SttError('Provider rejected the session or audio. Check key, model access, quota and language.')
        if self.provider == 'openai':
            item = event.get('item_id', '')
            if kind in ('input_audio_buffer.speech_started', 'input_audio_buffer.committed') and item not in self.order:
                self.order[item] = self.sequence
                self.sequence += 1
            if kind.endswith('.delta') or kind.endswith('.completed'):
                if item not in self.order:
                    self.order[item] = self.sequence
                    self.sequence += 1
                order = self.order[item]
                final = kind.endswith('.completed')
                text = event.get('transcript', '') if final else self.partials.get(item, '') + event.get('delta', '')
                self.partials[item] = text
                if final:
                    self.partials.pop(item, None)
                if len(self.order) > 128:
                    oldest = next(iter(self.order))
                    self.order.pop(oldest, None)
                    self.partials.pop(oldest, None)
                if order < self.latest:
                    return []
                self.latest = order
                return [{'type': 'final' if final else 'partial', 'text': text, 'segment': item}]
        elif self.provider == 'gemini':
            content = event.get('serverContent', {})
            result = []
            for field, status in [('interimInputTranscription', 'partial'), ('inputTranscription', 'final')]:
                if content.get(field, {}).get('text'):
                    result.append({'type': status, 'text': content[field]['text']})
            if event.get('goAway'):
                raise SttError('Provider session is ending. Stop and restart captions.')
            return result
        elif kind in ('partial_transcript', 'committed_transcript'):
            return [{'type': 'partial' if kind == 'partial_transcript' else 'final', 'text': event.get('text', '')}]
        return []
