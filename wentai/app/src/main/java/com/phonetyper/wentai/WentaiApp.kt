package com.phonetyper.wentai

import android.app.Application
import com.phonetyper.wentai.core.AppContainer

class WentaiApp : Application() {

    lateinit var container: AppContainer
        private set

    override fun onCreate() {
        super.onCreate()
        container = AppContainer(this)
    }
}
